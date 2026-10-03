/* The slab-tax loop in Java, in the three arithmetics worth comparing.
 *
 * One class with a mode argument rather than four files, because the empty
 * program has to start the same JVM as the timed ones for the subtraction to
 * mean anything. A separate Empty.class would load a different class and the
 * startup being subtracted would not be the startup being measured.
 *
 * N comes from argv for the reason the C file gives, and it matters more here:
 * C2 will happily hoist a loop whose bound it can prove constant.
 */
import java.math.BigDecimal;
import java.math.MathContext;

public class Tax {
    public static void main(String[] args) {
        if (args.length < 1) { System.err.println("need mode"); System.exit(2); }
        String mode = args[0];

        if (mode.equals("empty")) { System.out.println(0); return; }

        if (args.length < 2) { System.err.println("need N"); System.exit(2); }
        long n = Long.parseLong(args[1]);

        switch (mode) {
            case "bigdecimal" -> bigdecimal(n);
            case "long"       -> longPaisa(n);
            case "double"     -> doubles(n);
            default -> { System.err.println("unknown mode " + mode); System.exit(2); }
        }
    }

    /* Exact decimals — the arithmetic eTamil guarantees, and what a Java
     * program uses for money. */
    private static void bigdecimal(long n) {
        final BigDecimal RATE = new BigDecimal("0.05");
        final BigDecimal BASE = new BigDecimal(300000);

        BigDecimal total = BigDecimal.ZERO;
        for (long i = 0; i < n; i++) {
            BigDecimal income = BASE.add(BigDecimal.valueOf(i));
            BigDecimal tax = income.subtract(BASE).multiply(RATE);
            total = total.add(tax);
        }
        System.out.println(total);
    }

    /* Exact, using long scaled to paisa. */
    private static void longPaisa(long n) {
        long total = 0;                                   // paisa
        for (long i = 0; i < n; i++) {
            long income = 30000000L + i * 100L;           // paisa
            total += ((income - 30000000L) * 5L) / 100L;
        }
        System.out.printf("%d.%02d%n", total / 100, total % 100);
    }

    /* Binary float. Fast, and not exact for money. */
    private static void doubles(long n) {
        double total = 0.0;
        for (long i = 0; i < n; i++) {
            double income = 300000.0 + (double) i;
            double tax = (income - 300000.0) * 0.05;
            total += tax;
        }
        System.out.printf("%.2f%n", total);
    }
}
