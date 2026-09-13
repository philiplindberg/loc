<?php
// line
# hash line
#[Attribute]
/* block */
$s = "a // not"; // trailing
$t = 'b # not';
$h = <<<EOT
  # inside
  EOT;
?>
<!-- html comment -->
<p><?= $x ?></p>
<script>
  // js
</script>
