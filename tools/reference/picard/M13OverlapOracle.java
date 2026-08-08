import htsjdk.samtools.SAMRecord;
import htsjdk.samtools.SAMUtils;
import htsjdk.samtools.SamReader;
import htsjdk.samtools.SamReaderFactory;
import htsjdk.samtools.filter.SamRecordFilter;
import htsjdk.samtools.util.SamLocusIterator;
import htsjdk.samtools.util.SequenceUtil;

import java.io.File;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Narrow Milestone 13 differential oracle for the Picard 3.4.0 / HTSJDK 4.2.0
 * overlap semantics selected by ADR-0010.
 *
 * This is intentionally not a replacement implementation of CollectWgsMetrics or
 * CollectHsMetrics. It invokes the same pinned HTSJDK locus iterator and overlap helper and
 * reproduces only the surrounding filter/order operations needed to validate AlignGauge's
 * exact overlap primitives on the deterministic M13 fixture.
 */
public final class M13OverlapOracle {
    private static final int MINIMUM_MAPPING_QUALITY = 20;
    private static final int MINIMUM_BASE_QUALITY = 20;
    private static final int LOCUS_ACCUMULATION_CAP = 100_000;

    private M13OverlapOracle() {}

    public static void main(final String[] args) throws Exception {
        if (args.length != 1) {
            throw new IllegalArgumentException("usage: M13OverlapOracle <coordinate-sorted.bam>");
        }
        final File input = new File(args[0]);
        final WgsCounters wgs = collectWgsOverlap(input);
        final long hsOverlap = collectHsOverlap(input);

        System.out.printf("wgs_retained_bases\t%d%n", wgs.retainedBases);
        System.out.printf("wgs_baseq_excluded_bases\t%d%n", wgs.baseqExcludedBases);
        System.out.printf("wgs_overlap_excluded_bases\t%d%n", wgs.overlapExcludedBases);
        System.out.printf("hs_overlap_clipped_read_bases\t%d%n", hsOverlap);
    }

    private static WgsCounters collectWgsOverlap(final File input) throws Exception {
        final WgsCounters counters = new WgsCounters();
        try (SamReader reader = SamReaderFactory.makeDefault().open(input)) {
            final SamLocusIterator iterator = new SamLocusIterator(reader);
            iterator.setSamFilters(List.of(new PicardWgsFixtureFilter()));
            iterator.setIncludeNonPfReads(false);
            iterator.setMappingQualityScoreCutoff(0);
            iterator.setQualityScoreCutoff(0);
            iterator.setMaxReadsToAccumulatePerLocus(LOCUS_ACCUMULATION_CAP);

            for (final SamLocusIterator.LocusInfo locus : iterator) {
                final Set<String> readNames = new HashSet<>();
                for (final SamLocusIterator.RecordAndOffset observation :
                        locus.getRecordAndOffsets()) {
                    if (observation.getBaseQuality() < MINIMUM_BASE_QUALITY
                            || SequenceUtil.isNoCall(observation.getReadBase())) {
                        counters.baseqExcludedBases++;
                    } else if (!readNames.add(observation.getReadName())) {
                        counters.overlapExcludedBases++;
                    } else {
                        counters.retainedBases++;
                    }
                }
            }
            iterator.close();
        }
        return counters;
    }

    private static long collectHsOverlap(final File input) throws Exception {
        long overlap = 0;
        try (SamReader reader = SamReaderFactory.makeDefault().open(input)) {
            for (final SAMRecord record : reader) {
                if (record.isSecondaryAlignment()
                        || record.getReadFailsVendorQualityCheckFlag()
                        || record.getReadUnmappedFlag()
                        || record.getDuplicateReadFlag()
                        || record.getMappingQuality() < MINIMUM_MAPPING_QUALITY) {
                    continue;
                }
                overlap = Math.addExact(
                        overlap,
                        SAMUtils.getNumOverlappingAlignedBasesToClip(record));
            }
        }
        return overlap;
    }

    private static final class WgsCounters {
        long retainedBases;
        long baseqExcludedBases;
        long overlapExcludedBases;
    }

    /**
     * The fixture contains no adapter sequence, so this filter is the exact relevant subset of
     * CollectWgsMetrics' default record filters: secondary, MAPQ, duplicate, and paired/mate-mapped.
     * Non-PF handling remains delegated to SamLocusIterator, matching Picard.
     */
    private static final class PicardWgsFixtureFilter implements SamRecordFilter {
        @Override
        public boolean filterOut(final SAMRecord record) {
            return record.isSecondaryAlignment()
                    || record.getMappingQuality() < MINIMUM_MAPPING_QUALITY
                    || record.getDuplicateReadFlag()
                    || !record.getReadPairedFlag()
                    || record.getMateUnmappedFlag();
        }

        @Override
        public boolean filterOut(final SAMRecord first, final SAMRecord second) {
            return filterOut(first) || filterOut(second);
        }
    }
}
