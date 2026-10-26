' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expressions may reference module-level constants, procedure constants and enum members
' EXPECT: 30
' EXPECT: 25
' EXPECT: 11
Private Enum E
    First = 4
    Second
End Enum
Private Const Later As Long = Base * 2
Private Const Base As Long = 15
Public Sub Main()
    Const Local As Long = 5
    Const Mixed As Long = Local * Local
    Const FromEnum As Long = Second + 6
    Debug.Print CStr(Later)
    Debug.Print CStr(Mixed)
    Debug.Print CStr(FromEnum)
End Sub
