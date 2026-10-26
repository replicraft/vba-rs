' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Decimal effective type coerces the other operand to Decimal
' XFAIL: wrong not implemented: effective-type rules for comparisons involving Single/Currency/Decimal/LongLong and String
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim m As Variant
    m = CDec("0.1")
    Debug.Print CStr(m = 0.1)
    Debug.Print CStr(0.1 = m)
    Debug.Print CStr(m = "0.1")
    Debug.Print CStr(m < 1)
End Sub
