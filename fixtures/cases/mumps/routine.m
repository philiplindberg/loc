XUSDEMO ;SFISC/STAFF - DEMO ROUTINE ;11/14/2016
 ;;8.0;KERNEL;**1,2**;Jul 10, 1995
 ; A line whose first non-whitespace byte is ; is a comment.

EN ; a label and a comment make a code line
 N X S X="a;b" ; the first ; is inside a string
 S X="say ""hi;""" W X
 S Z="/*"
 ; the string above opened no block comment
 I '$D(^TMP("X")) S Y=X\2 Q
 D
 . ; a comment inside a dot block is code
 . Q
 Q
