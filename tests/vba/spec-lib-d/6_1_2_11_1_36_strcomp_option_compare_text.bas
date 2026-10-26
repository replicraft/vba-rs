' INV: vbal.6.1.2.11.1.36, func.strcomp
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp omitted Compare uses the module Option Compare
' EXPECT: 0
' EXPECT: 1
Option Compare Text
Public Sub Main()
    Debug.Print CStr(StrComp("a", "A"))
    Debug.Print CStr(StrComp("a", "A", vbBinaryCompare))
End Sub
