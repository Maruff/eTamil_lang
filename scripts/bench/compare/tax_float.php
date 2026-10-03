<?php
/* Binary float. Fast, and not exact for money. */
$n = (int)$argv[1];

$total = 0.0;
$i = 0;
while ($i < $n) {
    $income = 300000.0 + $i;
    $tax = ($income - 300000.0) * 0.05;
    $total = $total + $tax;
    $i++;
}

printf("%.2f\n", $total);
