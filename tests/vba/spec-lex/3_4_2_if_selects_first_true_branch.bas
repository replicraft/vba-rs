' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, at most one block is included: the first true one
' EXPECT: elseif1
Public Sub Main()
#If False Then
    Debug.Print "if"
#ElseIf True Then
    Debug.Print "elseif1"
#ElseIf True Then
    Debug.Print "elseif2"
#Else
    Debug.Print "else"
#End If
End Sub
