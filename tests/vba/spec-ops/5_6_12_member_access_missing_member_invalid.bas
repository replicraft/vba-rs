' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on a specific class without that member is invalid
' ERROR: load
'#MODULE Box class
Public W As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    Debug.Print CStr(b.Height)
End Sub
