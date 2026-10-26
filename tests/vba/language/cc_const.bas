' INV: cc.const, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation const and if directives
' EXPECT: two
#Const MODE = 2
#If MODE = 1 Then
Public Sub Main()
    Debug.Print "one"
End Sub
#ElseIf MODE = 2 Then
Public Sub Main()
    Debug.Print "two"
End Sub
#Else
Public Sub Main()
    Debug.Print "other"
End Sub
#End If
