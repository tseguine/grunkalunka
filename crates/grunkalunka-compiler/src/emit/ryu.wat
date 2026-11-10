;; WebAssembly adaptation of Ryū d2d (Ulf Adams, copyright 2018), as used by
;; ryu-js 1.0.3. Licensed under Apache-2.0, see ryu-LICENSE-APACHE.
;; For nonzero finite magnitudes, return the shortest decimal coefficient and
;; its power of ten. Ties select an even coefficient.

;; Upper half of a 64-by-64 unsigned product, using four 32-bit limbs.
(func $ryu_mul_high (param $a i64) (param $b i64) (result i64)
  (local $al i64) (local $bl i64) (local $ah i64) (local $bh i64)
  (local $t i64) (local $middle i64)
  (local.set $al (i64.and (local.get $a) (i64.const 0xffffffff)))
  (local.set $bl (i64.and (local.get $b) (i64.const 0xffffffff)))
  (local.set $ah (i64.shr_u (local.get $a) (i64.const 32)))
  (local.set $bh (i64.shr_u (local.get $b) (i64.const 32)))
  (local.set $t (i64.add (i64.mul (local.get $ah) (local.get $bl))
    (i64.shr_u (i64.mul (local.get $al) (local.get $bl)) (i64.const 32))))
  (local.set $middle (i64.add (i64.and (local.get $t) (i64.const 0xffffffff))
    (i64.mul (local.get $al) (local.get $bh))))
  (i64.add (i64.mul (local.get $ah) (local.get $bh))
    (i64.add (i64.shr_u (local.get $t) (i64.const 32))
             (i64.shr_u (local.get $middle) (i64.const 32)))))

;; Select the 64-bit window of m * (hi:lo) starting at bit j (64 < j < 128).
(func $ryu_scale (param $m i64) (param $lo i64) (param $hi i64) (param $j i32) (result i64)
  (local $add i64) (local $middle i64) (local $top i64)
  (local.set $add (call $ryu_mul_high (local.get $m) (local.get $lo)))
  (local.set $middle (i64.add (i64.mul (local.get $m) (local.get $hi)) (local.get $add)))
  (local.set $top (i64.add (call $ryu_mul_high (local.get $m) (local.get $hi))
    (i64.extend_i32_u (i64.lt_u (local.get $middle) (local.get $add)))))
  (i64.or (i64.shl (local.get $top) (i64.extend_i32_u (i32.sub (i32.const 128) (local.get $j))))
          (i64.shr_u (local.get $middle) (i64.extend_i32_u (i32.sub (local.get $j) (i32.const 64))))))

(func $ryu_divisible5 (param $v i64) (param $q i32) (result i32)
  (block $done (loop $factor
    (br_if $done (i32.eqz (local.get $q)))
    (if (i64.ne (i64.rem_u (local.get $v) (i64.const 5)) (i64.const 0))
      (then (return (i32.const 0))))
    (local.set $v (i64.div_u (local.get $v) (i64.const 5)))
    (local.set $q (i32.sub (local.get $q) (i32.const 1)))
    (br $factor)))
  (i32.const 1))

(func $ryu_decimal (param $fraction i64) (param $biased i32) (result i64 i32)
  (local $m i64) (local $e i32) (local $even i32) (local $symmetric i32)
  (local $q i32) (local $index i32) (local $j i32) (local $power i32)
  (local $table (ref null $ryu_table)) (local $lo i64) (local $hi i64)
  (local $center i64) (local $lower i64) (local $upper i64)
  (local $vr i64) (local $vm i64) (local $vp i64)
  (local $vm_zero i32) (local $vr_zero i32) (local $last i64)
  ;; Two extra binary places express both midpoints as integers. At normal
  ;; powers of two above the subnormal boundary the predecessor lies closer.
  (local.set $m (local.get $fraction))
  (local.set $e (i32.const -1076))
  (if (local.get $biased) (then
    (local.set $m (i64.or (local.get $m) (i64.const 0x10000000000000)))
    (local.set $e (i32.sub (local.get $biased) (i32.const 1077)))))
  (local.set $even (i64.eqz (i64.and (local.get $m) (i64.const 1))))
  (local.set $symmetric (i32.or (i64.ne (local.get $fraction) (i64.const 0))
                                (i32.le_u (local.get $biased) (i32.const 1))))
  (local.set $center (i64.shl (local.get $m) (i64.const 2)))
  (local.set $upper (i64.add (local.get $center) (i64.const 2)))
  (local.set $lower (i64.sub (i64.sub (local.get $center) (i64.const 1))
                             (i64.extend_i32_u (local.get $symmetric))))
  (if (i32.ge_s (local.get $e) (i32.const 0))
    (then
      ;; q = floor(log10(2^e)) - (e > 3)
      (local.set $q (i32.sub (i32.shr_u (i32.mul (local.get $e) (i32.const 78913)) (i32.const 18))
                             (i32.gt_s (local.get $e) (i32.const 3))))
      (local.set $power (local.get $q))
      (local.set $index (local.get $q))
      (local.set $table (global.get $inv5))
      (local.set $j (i32.add (i32.sub (local.get $q) (local.get $e))
        (i32.add (i32.const 125) (i32.shr_u (i32.mul (local.get $q) (i32.const 1217359)) (i32.const 19))))))
    (else
      ;; q = floor(log10(5^-e)) - (-e > 1)
      (local.set $index (i32.sub (i32.const 0) (local.get $e)))
      (local.set $q (i32.sub (i32.shr_u (i32.mul (local.get $index) (i32.const 732923)) (i32.const 20))
                             (i32.gt_u (local.get $index) (i32.const 1))))
      (local.set $power (i32.add (local.get $q) (local.get $e)))
      (local.set $index (i32.sub (local.get $index) (local.get $q)))
      (local.set $table (global.get $pow5))
      (local.set $j (i32.sub (i32.add (local.get $q) (i32.const 124))
        (i32.shr_u (i32.mul (local.get $index) (i32.const 1217359)) (i32.const 19))))))
  (local.set $index (i32.shl (local.get $index) (i32.const 1)))
  (local.set $lo (array.get $ryu_table (local.get $table) (local.get $index)))
  (local.set $hi (array.get $ryu_table (local.get $table) (i32.add (local.get $index) (i32.const 1))))
  (local.set $vr (call $ryu_scale (local.get $center) (local.get $lo) (local.get $hi) (local.get $j)))
  (local.set $vm (call $ryu_scale (local.get $lower) (local.get $lo) (local.get $hi) (local.get $j)))
  (local.set $vp (call $ryu_scale (local.get $upper) (local.get $lo) (local.get $hi) (local.get $j)))
  ;; Record which endpoints or center are exact. An odd significand cannot
  ;; use an endpoint, even if it otherwise permits a shorter spelling.
  (i32.ge_s (local.get $e) (i32.const 0))
  if
    (i32.le_u (local.get $q) (i32.const 21))
    if
      (i64.eqz (i64.rem_u (local.get $center) (i64.const 5)))
      if
        (local.set $vr_zero (call $ryu_divisible5 (local.get $center) (local.get $q)))
      else
        local.get $even
        if
          (local.set $vm_zero (call $ryu_divisible5 (local.get $lower) (local.get $q)))
        else
          (local.set $vp (i64.sub (local.get $vp)
            (i64.extend_i32_u (call $ryu_divisible5 (local.get $upper) (local.get $q)))))
        end
      end
    end
  else
    (i32.le_u (local.get $q) (i32.const 1))
    if
      (local.set $vr_zero (i32.const 1))
      local.get $even
      if
        (local.set $vm_zero (local.get $symmetric))
      else
        (local.set $vp (i64.sub (local.get $vp) (i64.const 1)))
      end
    else
      (i32.lt_u (local.get $q) (i32.const 63))
      if
        (local.set $vr_zero (i64.ge_u (i64.ctz (local.get $center)) (i64.extend_i32_u (local.get $q))))
      end
    end
  end
  ;; Shorten while a decimal lies inside the interval. Exact included lower
  ;; endpoints can lose a further trailing zero even when the quotients meet.
  (block $rounded (loop $shorten
    (br_if $rounded (i32.and
      (i64.le_u (i64.div_u (local.get $vp) (i64.const 10)) (i64.div_u (local.get $vm) (i64.const 10)))
      (i32.eqz (i32.and (local.get $vm_zero) (i64.eqz (i64.rem_u (local.get $vm) (i64.const 10)))))))
    (local.set $vm_zero (i32.and (local.get $vm_zero) (i64.eqz (i64.rem_u (local.get $vm) (i64.const 10)))))
    (local.set $vr_zero (i32.and (local.get $vr_zero) (i64.eqz (local.get $last))))
    (local.set $last (i64.rem_u (local.get $vr) (i64.const 10)))
    (local.set $vr (i64.div_u (local.get $vr) (i64.const 10)))
    (local.set $vm (i64.div_u (local.get $vm) (i64.const 10)))
    (local.set $vp (i64.div_u (local.get $vp) (i64.const 10)))
    (local.set $power (i32.add (local.get $power) (i32.const 1)))
    (br $shorten)))
  (if (i32.and (local.get $vr_zero) (i32.and (i64.eq (local.get $last) (i64.const 5))
                                             (i64.eqz (i64.and (local.get $vr) (i64.const 1)))))
    (then (local.set $last (i64.const 4))))
  (i64.add (local.get $vr) (i64.extend_i32_u (i32.or (i64.ge_u (local.get $last) (i64.const 5))
    (i32.and (i64.eq (local.get $vr) (local.get $vm)) (i32.eqz (i32.and (local.get $vm_zero) (local.get $even)))))))
  (local.get $power))
