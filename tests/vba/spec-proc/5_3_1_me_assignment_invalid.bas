' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 Me cannot be assigned to
' ERROR: load
'#MODULE Cls class
Public Sub Bad()
    Dim other As New Cls
    Set Me = other
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
