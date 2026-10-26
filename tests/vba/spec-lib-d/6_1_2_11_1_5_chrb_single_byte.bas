' INV: vbal.6.1.2.11.1.5
' SOURCE: spec ms-vbal 6.1.2.11.1.5 ChrB yields a single byte; Variant vs String result types
' XFAIL: unsupported ChrB yields a one-byte string
' EXPECT: a=1
' EXPECT: b=1
' EXPECT: t1=String
Public Sub Main()
    Debug.Print "a=" & CStr(LenB(ChrB(65)))
    Debug.Print "b=" & CStr(LenB(ChrB$(255)))
    Debug.Print "t1=" & TypeName(ChrB(65))
End Sub
