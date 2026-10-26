' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum members have different names
' ERROR: load
Private Enum Bad
    P = 1
    p = 2
End Enum
Public Sub Main()
End Sub
