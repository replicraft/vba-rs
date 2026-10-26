' INV: vbal.6.1.2.11.1.29, func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace return table
' EXPECT: a=[]
' EXPECT: b=[abc]
' EXPECT: c=[ac]
' EXPECT: d=[]
' EXPECT: e=[abc]
' EXPECT: f=True
' EXPECT: t=String
Public Sub Main()
    Debug.Print "a=[" & Replace("", "a", "b") & "]"
    Debug.Print "b=[" & Replace("abc", "", "x") & "]"
    Debug.Print "c=[" & Replace("abc", "b", "") & "]"
    Debug.Print "d=[" & Replace("abc", "b", "x", 4) & "]"
    Debug.Print "e=[" & Replace("abc", "b", "x", 1, 0) & "]"
    Dim s As String
    On Error Resume Next
    s = Replace(Null, "a", "b")
    Debug.Print "f=" & CStr(Err.Number <> 0)
    Debug.Print "t=" & TypeName(Replace("abc", "b", "x"))
End Sub
