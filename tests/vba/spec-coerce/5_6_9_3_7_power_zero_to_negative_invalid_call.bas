' INV: vbal.5.6.9.3.7, op.pow
' SOURCE: spec ms-vbal 5.6.9.3.7 ^ Operator, 0 raised to a negative power raises Invalid procedure call or argument
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: 0 ^ negative is error 11
' EXPECT: ok=0
' ERROR: 5
Public Sub Main()
    Dim z As Double
    Dim n As Double
    Dim r As Variant
    z = 0: n = 2
    r = z ^ n
    Debug.Print "ok=" & CStr(r)
    n = -1
    r = z ^ n
    Debug.Print "not reached"
End Sub
