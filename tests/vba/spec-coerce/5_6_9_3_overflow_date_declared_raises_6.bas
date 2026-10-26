' INV: vbal.5.6.9.3.2, op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 + Operator, overflow during coercion to Date with no Variant operand raises error 6
' EXPECT: ok
' ERROR: 6
Public Sub Main()
    Dim d As Date
    Dim e As Date
    d = 2958465
    e = d + 0
    Debug.Print "ok"
    e = d + 10
    Debug.Print "not reached"
End Sub
