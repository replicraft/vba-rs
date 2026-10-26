' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 public enum name distinct from other public enums in the project
' XFAIL: wrong not implemented: uniqueness of public Type/Enum names across modules and modules/types
' ERROR: load
Public Enum E
    A1 = 1
End Enum
Public Sub Main()
End Sub
'#MODULE Other standard
Public Enum E
    B1 = 2
End Enum
