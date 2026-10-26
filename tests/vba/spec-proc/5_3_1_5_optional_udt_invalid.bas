' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 declared type of an optional parameter may not be a UDT
' ERROR: load
Type Pt
    X As Long
End Type
Sub S(Optional p As Pt)
End Sub
Public Sub Main()
End Sub
