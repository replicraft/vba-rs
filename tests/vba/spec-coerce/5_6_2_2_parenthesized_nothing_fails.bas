' INV: vbal.5.6.2.2, vbal.5.6.6
' SOURCE: spec ms-vbal 5.6.2.2 Evaluation to a simple data value, Nothing raises error 91
' ERROR: 91
Public Sub Main()
    Dim o As Object
    Dim x As Variant
    x = (o)
    Debug.Print "not reached"
End Sub
