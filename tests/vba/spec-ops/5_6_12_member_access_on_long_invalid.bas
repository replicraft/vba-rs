' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on a Long declared type is invalid
' ERROR: load
Public Sub Main()
    Dim n As Long
    n = 1
    Debug.Print CStr(n.Foo)
End Sub
