' INV: vbal.5.6.3
' SOURCE: spec ms-vbal 5.6.3 Member resolution, a Nothing target raises error 91
' ERROR: 91
Public Sub Main()
    Dim o As Object
    Dim x As Variant
    x = o.Anything
    Debug.Print "not reached"
End Sub
