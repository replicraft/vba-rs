' INV: vbal.6.1.2.11.1.31
' SOURCE: derived ms-vbal 6.1.2.11.1.31 Right$ takes the rightmost characters like Right; a negative length is error 5
' EXPECT: a=[def]
' EXPECT: b=[]
' EXPECT: c=[abc]
' EXPECT: t=String
' EXPECT: err=5
Public Sub Main()
    Debug.Print "a=[" & Right$("abcdef", 3) & "]"
    Debug.Print "b=[" & Right$("abc", 0) & "]"
    Debug.Print "c=[" & Right$("abc", 10) & "]"
    Debug.Print "t=" & TypeName(Right$("abc", 1))
    On Error Resume Next
    Dim s As String
    s = Right$("abc", -1)
    Debug.Print "err=" & CStr(Err.Number)
End Sub
