' INV: vbal.5.6.9.2, op.add
' SOURCE: spec ms-vbal 5.6.9.2 Simple data operators, an object operand without a public default member raises error 438
' ERROR: 438
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Dim x As Variant
    Set o = New Plain
    x = o + 1
    Debug.Print "not reached"
End Sub
