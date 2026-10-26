' INV: vbal.5.6.9.3.2, op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 + Operator, a non-numeric String added to a number is coerced to Double and fails with Type mismatch
' ERROR: 13
Public Sub Main()
    Dim s As String
    Dim x As Variant
    s = "abc"
    x = s + 1
    Debug.Print "not reached"
End Sub
