#!/usr/bin/env bash
set -eo pipefail

no_std_packages=(
  mantle-alloy-consensus
  mantle-alloy-protocol
  mantle-alloy-genesis
  mantle-alloy-rpc-types
  mantle-alloy-rpc-types-engine
)

for package in "${no_std_packages[@]}"; do
  cmd="cargo +stable build -p $package --target riscv32imac-unknown-none-elf --no-default-features"
  if [ -n "$CI" ]; then
    echo "::group::$cmd"
  else
    printf "\n%s:\n  %s\n" "$package" "$cmd"
  fi

  $cmd

  if [ -n "$CI" ]; then
    echo "::endgroup::"
  fi
done
