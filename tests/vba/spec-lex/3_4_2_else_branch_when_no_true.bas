' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, the #Else block is included when no expression is true
' EXPECT: else
Public Sub Main()
#If False Then
    Debug.Print "if"
#ElseIf False Then
    Debug.Print "elseif"
#Else
    Debug.Print "else"
#End If
End Sub
