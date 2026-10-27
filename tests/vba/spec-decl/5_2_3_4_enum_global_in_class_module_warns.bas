' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 Global Enum in a class module (spec-strict: a warning by default, the program runs)
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(1)
End Sub
'#MODULE C class
Global Enum E
    A1 = 1
End Enum
