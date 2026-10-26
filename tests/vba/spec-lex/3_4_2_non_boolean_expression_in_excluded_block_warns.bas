' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, cc-expression rules apply even when the cc-if-block is in an excluded block (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
#If False Then
  #If "abc" Then
    Debug.Print "x"
  #End If
#End If
    Debug.Print "ok"
End Sub
