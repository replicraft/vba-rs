' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational effective value type Byte with String or Empty
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim b As Byte, e As Variant
    b = 5
    Debug.Print CStr(b = "5")
    Debug.Print CStr(b < "10")
    Debug.Print CStr("10" > b)
    Debug.Print CStr(CByte(0) = e)
    Debug.Print CStr(e < b)
End Sub
