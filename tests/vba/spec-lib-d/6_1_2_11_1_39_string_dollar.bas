' INV: vbal.6.1.2.11.1.39
' SOURCE: derived ms-vbal 6.1.2.11.1.39 String$ repeats the first character of a string or the character with the given code
' EXPECT: a=xxx
' EXPECT: b=aaa
' EXPECT: c=AA
' EXPECT: d=[]
' EXPECT: t=String
' EXPECT: err=5
Public Sub Main()
    Debug.Print "a=" & String$(3, "x")
    Debug.Print "b=" & String$(3, "abc")
    Debug.Print "c=" & String$(2, 65)
    Debug.Print "d=[" & String$(0, "x") & "]"
    Debug.Print "t=" & TypeName(String$(1, "x"))
    On Error Resume Next
    Dim s As String
    s = String$(-1, "x")
    Debug.Print "err=" & CStr(Err.Number)
End Sub
