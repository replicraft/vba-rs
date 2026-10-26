' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 module variable names must be distinct
' ERROR: load
Private x As Long
Private x As String
Public Sub Main()
End Sub
