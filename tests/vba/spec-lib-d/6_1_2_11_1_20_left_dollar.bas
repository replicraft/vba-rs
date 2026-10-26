' INV: vbal.6.1.2.11.1.20, func.left
' SOURCE: derived ms-vbal 6.1.2.11.1.20 Left$ takes the leftmost characters like Left; a negative length is error 5
' EXPECT: a=[abc]
' EXPECT: b=[]
' EXPECT: c=[abc]
' EXPECT: t=String
' EXPECT: err=5
Public Sub Main()
    Debug.Print "a=[" & Left$("abcdef", 3) & "]"
    Debug.Print "b=[" & Left$("abc", 0) & "]"
    Debug.Print "c=[" & Left$("abc", 10) & "]"
    Debug.Print "t=" & TypeName(Left$("abc", 1))
    On Error Resume Next
    Dim s As String
    s = Left$("abc", -1)
    Debug.Print "err=" & CStr(Err.Number)
End Sub
