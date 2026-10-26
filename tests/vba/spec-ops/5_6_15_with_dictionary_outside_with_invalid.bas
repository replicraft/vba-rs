' INV: vbal.5.6.15, stmt.with
' SOURCE: spec ms-vbal 5.6.15 a With dictionary access outside any With block is invalid
' ERROR: load
Public Sub Main()
    Debug.Print CStr(!key)
End Sub
