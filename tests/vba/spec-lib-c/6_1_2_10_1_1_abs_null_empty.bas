' INV: vbal.6.1.2.10.1.1, func.abs
' SOURCE: spec ms-vbal 6.1.2.10.1.1 Abs of Null is Null, of Empty is Integer 0
' EXPECT: null=True
' EXPECT: Integer 0
Public Sub Main()
    Dim n As Variant, e As Variant, r As Variant
    n = Null
    Debug.Print "null=" & IsNull(Abs(n))
    r = Abs(e)
    Debug.Print TypeName(r) & " " & CStr(r)
End Sub
