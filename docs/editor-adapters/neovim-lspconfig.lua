-- Canonical Neovim adapter path for Kujo LSP.
-- Requires nvim-lspconfig.

require('lspconfig').kujo_lsp = {
  default_config = {
    cmd = { 'kujo', 'lsp' },
    filetypes = { 'kujo' },
    root_dir = function(fname)
      return require('lspconfig.util').find_git_ancestor(fname)
        or require('lspconfig.util').path.dirname(fname)
    end,
    single_file_support = true,
  },
}

require('lspconfig').kujo_lsp.setup({})
