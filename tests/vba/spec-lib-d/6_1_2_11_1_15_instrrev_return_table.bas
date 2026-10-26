' INV: vbal.6.1.2.11.1.15, func.instrrev
' SOURCE: spec ms-vbal 6.1.2.11.1.15 InStrRev return table
' EXPECT: a=0
' EXPECT: c=2
' EXPECT: e=0
' EXPECT: f=5
' EXPECT: g=2
' EXPECT: h=Long
' EXPECT: i=2
Public Sub Main()
    Debug.Print "a=" & CStr(InStrRev("", "a"))
    Debug.Print "c=" & CStr(InStrRev("abc", "", 2))
    Debug.Print "e=" & CStr(InStrRev("abc", "z"))
    Debug.Print "f=" & CStr(InStrRev("abcabc", "b"))
    Debug.Print "g=" & CStr(InStrRev("abcabc", "b", 4))
    Debug.Print "h=" & TypeName(InStrRev("abc", "b"))
    Debug.Print "i=" & CStr(InStrRev("ABCABC", "b", -1, vbTextCompare) - 3)
End Sub
