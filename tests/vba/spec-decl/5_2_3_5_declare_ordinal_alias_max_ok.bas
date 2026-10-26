' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 ordinal alias 32,767 is in range
' EXPECT: ok
Private Declare PtrSafe Function Edge Lib "somelib" Alias "#32767" () As Long
Private Declare PtrSafe Function Zero Lib "somelib" Alias "#0" () As Long
Public Sub Main()
    Debug.Print "ok"
End Sub
