' INV: vbal.5.6.9.3
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators runtime semantics, an array value operand raises Type mismatch
' ERROR: 13
Public Sub Main()
    Dim a(1) As Long
    Dim v As Variant
    Dim x As Variant
    v = a
    x = v + 1
    Debug.Print "not reached"
End Sub
