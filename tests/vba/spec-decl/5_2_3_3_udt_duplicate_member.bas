' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 UDT member names must be distinct
' ERROR: load
Private Type Rec
    a As Long
    A As String
End Type
Public Sub Main()
End Sub
