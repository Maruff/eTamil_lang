<?php
/* Exact, using int scaled to paisa. PHP has no decimal type, so this is what a
 * PHP program does when it has to be right about money and bcmath is too slow.
 *
 * PHP_INT_SIZE is 8 here, so the running total stays exact in int64 — the same
 * guarantee the C and Python int rows give. N comes from argv for the reason
 * the C file gives. */
$n = (int)$argv[1];

$total = 0;                                   // paisa
$i = 0;
while ($i < $n) {
    $income = 30000000 + $i * 100;            // paisa
    $total += intdiv(($income - 30000000) * 5, 100);
    $i++;
}

printf("%d.%02d\n", intdiv($total, 100), $total % 100);
