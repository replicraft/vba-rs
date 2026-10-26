' INV: vbal.5.6.14
' SOURCE: spec ms-vbal 5.6.14 dictionary access invalid when the declared type is not a class, Object or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long
    n = 1
    Debug.Print CStr(n!key)
End Sub
