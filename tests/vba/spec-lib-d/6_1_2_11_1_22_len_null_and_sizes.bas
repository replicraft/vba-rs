' INV: vbal.6.1.2.11.1.22, func.len
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len/LenB: characters in a string, Null gives Null, Variant treated as String, byte size of numeric variables
' XFAIL: wrong pending
' EXPECT: a=3
' EXPECT: b=0
' EXPECT: c=True
' EXPECT: d=True
' EXPECT: e=5
' EXPECT: i=2
' EXPECT: l=4
' EXPECT: dbl=8
' EXPECT: by=1
' EXPECT: lenb=4
Public Sub Main()
    Dim v As Variant, i As Integer, l As Long, d As Double, b As Byte
    Debug.Print "a=" & CStr(Len("abc"))
    Debug.Print "b=" & CStr(Len(""))
    Debug.Print "c=" & CStr(IsNull(Len(Null)))
    Debug.Print "d=" & CStr(IsNull(LenB(Null)))
    v = "hello"
    Debug.Print "e=" & CStr(Len(v))
    Debug.Print "i=" & CStr(Len(i))
    Debug.Print "l=" & CStr(Len(l))
    Debug.Print "dbl=" & CStr(Len(d))
    Debug.Print "by=" & CStr(Len(b))
    Debug.Print "lenb=" & CStr(LenB(l))
End Sub
