VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Account"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = True
Option Explicit

Implements IPrintable

Public Event BalanceChanged(ByVal oldValue As Currency, ByVal newValue As Currency)
Public Event Overdrawn()

Private WithEvents mTimer As Timer
Private mBalance As Currency
Private mOwner As String
Private mHistory As Collection

Private Sub Class_Initialize()
    Set mHistory = New Collection
    mBalance = 0
End Sub

Private Sub Class_Terminate()
    Set mHistory = Nothing
End Sub

Public Property Get Balance() As Currency
Attribute Balance.VB_Description = "Current balance"
Attribute Balance.VB_UserMemId = 0
    Balance = mBalance
End Property

Public Property Let Owner(ByVal value As String)
    mOwner = value
End Property

Public Property Get Owner() As String
    Owner = mOwner
End Property

Public Property Set History(ByVal value As Collection)
    Set mHistory = value
End Property

Public Sub Deposit(ByVal amount As Currency)
    Dim old As Currency
    If amount <= 0 Then Err.Raise 5, , "Amount must be positive"
    old = mBalance
    mBalance = mBalance + amount
    mHistory.Add amount
    RaiseEvent BalanceChanged(old, mBalance)
End Sub

Public Sub Withdraw(ByVal amount As Currency)
    If amount > mBalance Then
        RaiseEvent Overdrawn
        Exit Sub
    End If
    mBalance = mBalance - amount
End Sub

Private Function IPrintable_Describe() As String
    IPrintable_Describe = mOwner & ": " & Format$(mBalance, "#,##0.00")
End Function

Private Sub mTimer_Tick()
    Debug.Print "tick"
End Sub

Friend Function Internal(Optional ByVal x As Long = -1) As Boolean
    Internal = (x < 0)
End Function

Public Function NewAccount(ByVal owner As String) As Account
    Dim a As New Account
    a.Owner = owner
    Set NewAccount = a
End Function
