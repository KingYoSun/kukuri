import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const SOURCE_ROOT = path.resolve(process.cwd(), 'src');
const PRODUCT_ROOTS = ['components', 'shell'].map((directory) => path.join(SOURCE_ROOT, directory));
const DIRECT_ICON_BUTTON_PATTERN = /<Button\b[^>]*\bsize=['"]icon['"][^>]*>/g;
const TOOLTIP_WRAPPERS = new Set(['IconButtonTooltip', 'ReactionTooltipButton']);

function productTsxFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) return productTsxFiles(entryPath);
    if (
      !entry.name.endsWith('.tsx') ||
      entry.name.endsWith('.test.tsx') ||
      entry.name.endsWith('.stories.tsx')
    ) {
      return [];
    }
    return [entryPath];
  });
}

function jsxTagName(node: ts.JsxTagNameExpression, sourceFile: ts.SourceFile) {
  return node.getText(sourceFile);
}

function hasTrueAttribute(
  node: ts.JsxOpeningLikeElement,
  name: string,
  sourceFile: ts.SourceFile
) {
  return node.attributes.properties.some((attribute) => {
    if (!ts.isJsxAttribute(attribute) || attribute.name.getText(sourceFile) !== name) return false;
    if (!attribute.initializer) return true;
    return attribute.initializer.getText(sourceFile) === "'true'";
  });
}

function hasTooltipAncestor(node: ts.Node, sourceFile: ts.SourceFile) {
  let current = node.parent;
  while (current) {
    if (
      ts.isJsxElement(current) &&
      TOOLTIP_WRAPPERS.has(jsxTagName(current.openingElement.tagName, sourceFile))
    ) {
      return true;
    }
    current = current.parent;
  }
  return false;
}

function isIconLikeChild(
  child: ts.JsxChild,
  lucideNames: Set<string>,
  sourceFile: ts.SourceFile
): boolean {
  if (ts.isJsxText(child)) return child.getText(sourceFile).trim().length === 0;
  if (ts.isJsxExpression(child)) return !child.expression;
  if (ts.isJsxFragment(child)) {
    return child.children.every((nestedChild) =>
      isIconLikeChild(nestedChild, lucideNames, sourceFile)
    );
  }
  const opening = ts.isJsxElement(child) ? child.openingElement : child;
  const tagName = jsxTagName(opening.tagName, sourceFile);
  return lucideNames.has(tagName) || hasTrueAttribute(opening, 'aria-hidden', sourceFile);
}

function uncoveredNativeIconButtons(filePath: string) {
  const source = readFileSync(filePath, 'utf8');
  const sourceFile = ts.createSourceFile(
    filePath,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX
  );
  const lucideNames = new Set<string>();
  sourceFile.statements.forEach((statement) => {
    if (
      ts.isImportDeclaration(statement) &&
      statement.moduleSpecifier.getText(sourceFile) === "'lucide-react'" &&
      statement.importClause?.namedBindings &&
      ts.isNamedImports(statement.importClause.namedBindings)
    ) {
      statement.importClause.namedBindings.elements.forEach((element) => {
        lucideNames.add(element.name.text);
      });
    }
  });

  const violations: string[] = [];
  const visit = (node: ts.Node) => {
    if (ts.isJsxElement(node) && jsxTagName(node.openingElement.tagName, sourceFile) === 'button') {
      const iconChildren = node.children.filter((child) => isIconLikeChild(child, lucideNames, sourceFile));
      const contentChildren = node.children.filter(
        (child) => !isIconLikeChild(child, lucideNames, sourceFile)
      );
      if (iconChildren.length > 0 && contentChildren.length === 0 && !hasTooltipAncestor(node, sourceFile)) {
        const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
        violations.push(`${path.relative(SOURCE_ROOT, filePath)}:${line}`);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return violations;
}

describe('アイコン専用ボタンのtooltip coverage', () => {
  it('共通IconButton以外でButton size=iconを直接使わない', () => {
    const violations = PRODUCT_ROOTS.flatMap(productTsxFiles).flatMap((filePath) => {
      if (filePath.endsWith(path.join('ui', 'icon-button.tsx'))) return [];
      const source = readFileSync(filePath, 'utf8');
      return [...source.matchAll(DIRECT_ICON_BUTTON_PATTERN)].map((match) => {
        const line = source.slice(0, match.index).split(/\r?\n/).length;
        return `${path.relative(SOURCE_ROOT, filePath)}:${line}`;
      });
    });

    expect(violations).toEqual([]);
  });

  it('Lucide iconまたはaria-hiddenの印だけを持つnative buttonをtooltipで包む', () => {
    const violations = PRODUCT_ROOTS.flatMap(productTsxFiles).flatMap(uncoveredNativeIconButtons);
    expect(violations).toEqual([]);
  });
});
