' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, #ElseIf blocks must precede the #Else block in a cc-if-block
' ERROR: load
Public Sub Main()
#If True Then
    Debug.Print "x"
#Else
    Debug.Print "y"
#ElseIf True Then
    Debug.Print "z"
#End If
End Sub
