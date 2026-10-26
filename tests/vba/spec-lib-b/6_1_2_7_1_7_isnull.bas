' INV: vbal.6.1.2.7.1.7, func.isnull
' SOURCE: spec ms-vbal 6.1.2.7.1.7 IsNull true only for the Null data value
' EXPECT: null=True
' EXPECT: variant=True
' EXPECT: empty=False
' EXPECT: emptystr=False
' EXPECT: zero=False
Public Sub Main()
    Dim v As Variant
    v = Null
    Debug.Print "null=" & CStr(IsNull(Null))
    Debug.Print "variant=" & CStr(IsNull(v))
    Debug.Print "empty=" & CStr(IsNull(Empty))
    Debug.Print "emptystr=" & CStr(IsNull(""))
    Debug.Print "zero=" & CStr(IsNull(0))
End Sub
