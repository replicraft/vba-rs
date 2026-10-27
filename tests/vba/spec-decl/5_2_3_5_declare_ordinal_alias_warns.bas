' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 ordinal alias out of range (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Private Declare PtrSafe Function Bad Lib "somelib" Alias "#32768" () As Long
Public Sub Main()
    Debug.Print "ok"
End Sub
