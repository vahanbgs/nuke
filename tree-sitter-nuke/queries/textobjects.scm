(field) @entry.around
(field value: (_) @entry.inside)

(entry) @entry.around
(entry value: (_) @entry.inside)

(binding) @entry.around
(binding value: (_) @entry.inside)

(comment) @comment.inside
(comment)+ @comment.around
