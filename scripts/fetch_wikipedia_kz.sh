#!/usr/bin/env bash
# Download Kazakh Wikipedia XML dump and extract plain article text.
#
# Output: data/external/wikipedia_kz_plain.txt
#   One article per block, articles separated by "\x1e" (ASCII record separator).
#   Within an article, text is space-normalized MediaWiki source minus markup.
#
# License: CC-BY-SA 4.0 (Wikipedia articles)
# Attribution: "Wikipedia contributors, Kazakh Wikipedia
#               (https://kk.wikipedia.org), CC-BY-SA 4.0"
#
# Note: data/external/ is gitignored. The 155 MB compressed dump is not committed.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

archive="kkwiki-latest-pages-articles.xml.bz2"
url="https://dumps.wikimedia.org/kkwiki/latest/$archive"
out_file="wikipedia_kz_plain.txt"

if [[ ! -f "$archive" ]]; then
  echo "downloading $url (~155 MB)"
  curl -L -o "$archive" "$url"
else
  echo "reusing existing $archive"
fi

echo "streaming extraction (bzcat + perl; articles separated by RS 0x1e)..."

# Perl one-liner:
#   * tracks whether we're inside a <text xml:space="preserve">...</text> block
#   * strips common MediaWiki markup:
#       {{...}}           templates
#       [[File:...]]      file links (incl. nested)
#       [[foo|bar]]       piped links -> bar
#       [[foo]]           simple links -> foo
#       '''bold''', ''italic''
#       <ref>...</ref>    references
#       <!-- ... -->      comments
#       <...>             remaining HTML/XML tags
#       ==headings==      stripped
#   * emits plain text separated by RS between articles
perl_script="$out_dir/_extract_wikipedia_kz.pl"
cat > "$perl_script" <<'PERL'
use strict;
use warnings;
use utf8;
binmode(STDIN, ":utf8");
binmode(STDOUT, ":utf8");

my $in_text = 0;
my $buf = "";
my $RS_CHAR = chr(0x1e);

while (my $line = <STDIN>) {
    if ($line =~ /<text[^>]*>/) {
        $in_text = 1;
        $line =~ s{.*?<text[^>]*>}{}s;
    }
    next unless $in_text;

    if ($line =~ m{</text>}) {
        $line =~ s{</text>.*}{}s;
        $buf .= $line;

        # Strip MediaWiki markup, applied in passes so nested templates collapse.
        for my $_pass (1..4) {
            $buf =~ s/\{\{[^{}]*\}\}//g;
        }
        $buf =~ s/\[\[File:[^\[\]]*\]\]//gi;
        $buf =~ s/\[\[Сурет:[^\[\]]*\]\]//gi;
        $buf =~ s/\[\[([^\[\]|]+)\|([^\[\]]+)\]\]/$2/g;
        $buf =~ s/\[\[([^\[\]]+)\]\]/$1/g;
        $buf =~ s/<ref[^>]*\/>//g;
        $buf =~ s/<ref[^>]*>.*?<\/ref>//gs;
        $buf =~ s/<!--.*?-->//gs;
        $buf =~ s/<[^>]+>//g;
        $buf =~ s/'{2,5}//g;
        $buf =~ s/^={2,}.*?={2,}$//mg;
        $buf =~ s/^[\*#]+//mg;
        $buf =~ s/\s+/ /g;
        $buf =~ s/^\s+|\s+$//g;

        if (length($buf) > 100) {
            print $buf, $RS_CHAR;
        }
        $in_text = 0;
        $buf = "";
    } else {
        $buf .= $line;
    }
}
PERL

bzcat "$archive" | perl "$perl_script" > "$out_file"
rm -f "$perl_script"

size=$(wc -c < "$out_file")
articles=$(tr -cd $'\x1e' < "$out_file" | wc -c)
echo "wrote $out_file: $articles articles, $size bytes"
