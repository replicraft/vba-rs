' INV: stmt.declare
' SOURCE: unverified current behavior: the portable Win32 shim answers GetTickCount deterministically (0 under the default manual clock); not checked against Windows
' EXPECT:  0 
' GRANT: native-declare
Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
Public Sub Main()
    Debug.Print GetTickCount()
End Sub
