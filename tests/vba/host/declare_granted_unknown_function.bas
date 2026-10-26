' INV: stmt.declare
' SOURCE: derived a Declare the host cannot resolve is error 48 (File not found), never a silent success
' ERROR: bridge
' GRANT: native-declare
Declare PtrSafe Function NoSuchFn Lib "nolib" () As Long
Public Sub Main()
    Debug.Print NoSuchFn()
End Sub
