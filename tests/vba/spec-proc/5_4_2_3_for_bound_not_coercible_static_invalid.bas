' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 start, end and step must be statically Let-coercible to Double
' ERROR: load
Type Pt
    X As Long
End Type
Public Sub Main()
    Dim i As Long
    Dim p As Pt
    For i = 1 To p
    Next i
End Sub
