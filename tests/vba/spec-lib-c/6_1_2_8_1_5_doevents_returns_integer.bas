' INV: vbal.6.1.2.8.1.5, func.doevents
' SOURCE: spec ms-vbal 6.1.2.8.1.5 DoEvents declared to return Integer
' EXPECT: Integer
Public Sub Main()
    Dim r As Variant
    r = DoEvents()
    Debug.Print TypeName(r)
End Sub
