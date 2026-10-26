' INV: vbal.5.6.9.2, op.add
' SOURCE: spec ms-vbal 5.6.9.2 Simple data operators, a Nothing operand raises error 91
' ERROR: 91
Public Sub Main()
    Dim o As Object
    Dim x As Variant
    x = o + 1
    Debug.Print "not reached"
End Sub
