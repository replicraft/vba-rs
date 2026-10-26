' INV: vbal.6.1.2.3.1.20, func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str(Null) returns Null
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Str(Null) is error 94
' EXPECT: null=True
Public Sub Main()
    Dim v As Variant
    v = Str(Null)
    Debug.Print "null=" & CStr(IsNull(v))
End Sub
