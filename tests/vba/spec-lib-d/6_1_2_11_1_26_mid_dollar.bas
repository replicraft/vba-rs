' INV: vbal.6.1.2.11.1.26, func.mid
' SOURCE: derived ms-vbal 6.1.2.11.1.26 Mid$ extracts from a 1-based start; start 0 or a negative length is error 5
' EXPECT: a=[bcd]
' EXPECT: b=[def]
' EXPECT: c=[]
' EXPECT: t=String
' EXPECT: e0=5
' EXPECT: en=5
Public Sub Main()
    Debug.Print "a=[" & Mid$("abcdef", 2, 3) & "]"
    Debug.Print "b=[" & Mid$("abcdef", 4) & "]"
    Debug.Print "c=[" & Mid$("abc", 5) & "]"
    Debug.Print "t=" & TypeName(Mid$("abc", 1))
    Dim s As String
    On Error Resume Next
    s = Mid$("abc", 0)
    Debug.Print "e0=" & CStr(Err.Number)
    Err.Clear
    s = Mid$("abc", 2, -1)
    Debug.Print "en=" & CStr(Err.Number)
End Sub
