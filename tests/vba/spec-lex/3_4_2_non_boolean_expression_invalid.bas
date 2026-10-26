' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, every cc-expression MUST be Let-coercible to Boolean
' ERROR: load
Public Sub Main()
#If "abc" Then
    Debug.Print "x"
#End If
End Sub
