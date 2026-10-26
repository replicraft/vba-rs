' INV: vbal.5.6.3
' SOURCE: spec ms-vbal 5.6.3 Member resolution, a target data value that is not a class, UDT or Nothing raises error 424
' ERROR: 424
Public Sub Main()
    Dim v As Variant
    Dim x As Variant
    v = 5
    x = v.Anything
    Debug.Print "not reached"
End Sub
