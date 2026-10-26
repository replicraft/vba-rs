' INV: vbal.5.6.9.3.3, op.sub
' SOURCE: spec ms-vbal 5.6.9.3.3 Binary - operator, Byte - Byte below 0 is outside the Byte range: error 6
' EXPECT: ok=0
' ERROR: 6
Public Sub Main()
    Dim y As Byte, z As Byte
    Dim r As Variant
    y = 3: z = 3
    r = y - z
    Debug.Print "ok=" & CStr(r)
    z = 4
    r = y - z
    Debug.Print "not reached"
End Sub
