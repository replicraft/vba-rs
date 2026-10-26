' INV: vbal.5.2.3.1.4
' SOURCE: spec ms-vbal 5.2.3.1.4 string length MUST be at most 65,526
' ERROR: load
Dim s As String * 65527
Public Sub Main()
End Sub
