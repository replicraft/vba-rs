' INV: op.mod
' SOURCE: unverified sign of the dividend; behavior of the implementation
' EXPECT: -1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(-7 Mod 3)
    Debug.Print CStr(7 Mod -3)
End Sub
