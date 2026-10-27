' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 public enum name distinct from other public enums in the project (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Enum E
    A1 = 1
End Enum
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE Other standard
Public Enum E
    B1 = 2
End Enum
