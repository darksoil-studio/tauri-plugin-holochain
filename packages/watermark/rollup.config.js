import typescript from "@rollup/plugin-typescript";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import terser from "@rollup/plugin-terser";
import analyzer from "rollup-plugin-analyzer";
import css from "rollup-plugin-import-css";

export default {
  input: "src/index.ts",
  output: {
    dir: "dist",
    format: "es",
  },
  plugins: [
    css({
      inject: true,
    }),
    commonjs(),
    resolve({
      browser: true,
    }),
    typescript(),
    // terser(),
    analyzer(),
  ],
};
