' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, a directive line may end with a single-quote comment (cc-eol)
' EXPECT: a
#Const X = 1 ' comment
#If X = 1 Then ' comment
Public Sub Main()
    Debug.Print "a"
End Sub
#Else ' comment
Public Sub Main()
    Debug.Print "b"
End Sub
#End If ' comment
