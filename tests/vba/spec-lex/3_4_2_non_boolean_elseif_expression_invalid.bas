' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, an #ElseIf expression is checked even when an earlier branch is selected
' ERROR: load
Public Sub Main()
#If True Then
    Debug.Print "x"
#ElseIf "abc" Then
    Debug.Print "y"
#End If
End Sub
