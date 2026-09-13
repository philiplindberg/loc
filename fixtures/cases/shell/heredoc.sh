cat <<EOF
# inside
EOF
cat <<-'EOF' > out
	# inside
	EOF
cat << EOF
# spaced
EOF
cat <<< "$x" # herestring
