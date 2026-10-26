' INV: vbal.6.1.2.11.1.40, func.strreverse
' SOURCE: spec ms-vbal 6.1.2.11.1.40 StrReverse reverses; zero-length stays; Null is an error
' EXPECT: a=cba
' EXPECT: b=[]
' EXPECT: c=a
' EXPECT: n=True
Public Sub Main()
    Debug.Print "a=" & StrReverse("abc")
    Debug.Print "b=[" & StrReverse("") & "]"
    Debug.Print "c=" & StrReverse("a")
    Dim s As Variant
    On Error Resume Next
    s = StrReverse(Null)
    Debug.Print "n=" & CStr(Err.Number <> 0)
End Sub
