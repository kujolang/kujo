;;; kujo-eglot.el --- Minimal Kujo mode and Eglot adapter -*- lexical-binding: t; -*-

(define-derived-mode kujo-mode prog-mode "Kujo"
  "Minimal major mode for Kujo source files.")

(add-to-list 'auto-mode-alist '("\\.kujo\\'" . kujo-mode))

(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((kujo-mode :language-id "kujo") . ("kujo" "lsp"))))

;; Remove this hook if you prefer to start the server manually with M-x eglot.
(add-hook 'kujo-mode-hook #'eglot-ensure)

(provide 'kujo-eglot)
;;; kujo-eglot.el ends here
